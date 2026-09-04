use crate::config::*;
use crate::font::FontManager;
use crate::layout::{calc_thumb, compute_layout, ViewportLayout};
use crate::sidebar::{Sidebar, MENU_ITEMS};
use crate::tabs::TabManager;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::window::Window;

pub struct Renderer {
    _ctx: Context<Arc<Window>>,
    surface: Surface<Arc<Window>, Arc<Window>>,
    pub width: usize,
    pub height: usize,
    pub font_manager: FontManager,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Self {
        let ctx = Context::new(window.clone()).expect("Failed to init softbuffer");
        let mut surface = Surface::new(&ctx, window.clone()).expect("Failed to create surface");
        let size = window.inner_size();
        let w = size.width.max(1);
        let h = size.height.max(1);
        if let (Some(wnz), Some(hnz)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            surface
                .resize(wnz, hnz)
                .expect("Failed to set surface size");
        }
        Self {
            _ctx: ctx,
            surface,
            width: w as usize,
            height: h as usize,
            font_manager: FontManager::new(),
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.width = w as usize;
        self.height = h as usize;
        if let (Some(wnz), Some(hnz)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            self.surface
                .resize(wnz, hnz)
                .expect("Failed to resize surface");
        }
    }

    pub fn layout(&self, total_lines: usize, sidebar_w: usize) -> ViewportLayout {
        compute_layout(
            self.width,
            self.height,
            self.font_manager.char_width,
            self.font_manager.line_height,
            total_lines,
            sidebar_w,
        )
    }

    pub fn render(&mut self, tabs: &TabManager, sidebar: &Sidebar) {
        if self.width == 0 || self.height == 0 {
            return;
        }

        let screen_w = self.width;
        let screen_h = self.height;
        let total_lines = tabs
            .active_tab()
            .map(|t| t.buffer.text().len_lines())
            .unwrap_or(0);
        let layout = self.layout(total_lines, sidebar.width);

        let mut frame = self.surface.buffer_mut().expect("Failed to get buffer");
        frame.fill(COLOR_BACKGROUND);

        let cw = self.font_manager.char_width;
        let lh = self.font_manager.line_height;
        let row_offset_y = (SIDEBAR_ROW_HEIGHT.saturating_sub(lh)) / 2;

        draw_solid_rect(
            &mut frame,
            screen_w,
            screen_h,
            0,
            0,
            sidebar.width,
            screen_h,
            COLOR_SIDEBAR_BG,
        );
        draw_solid_rect(
            &mut frame,
            screen_w,
            screen_h,
            sidebar.width - 1,
            0,
            1,
            screen_h,
            COLOR_SIDEBAR_BORDER,
        );

        let total_sidebar_h = sidebar.total_content_height();
        let has_sidebar_scroll = total_sidebar_h > screen_h;
        let max_text_x = if has_sidebar_scroll {
            sidebar.width.saturating_sub(SCROLLBAR_THICKNESS + 4)
        } else {
            sidebar.width.saturating_sub(8)
        };

        let total_menu_h = sidebar.menu_total_height();
        let menu_screen_y = -(sidebar.scroll_y as i32);

        draw_solid_rect_i32(
            &mut frame,
            screen_w,
            screen_h,
            0,
            menu_screen_y,
            sidebar.width - 1,
            total_menu_h,
            COLOR_BACKGROUND,
        );

        let menu_header_bg = if sidebar.hovered_menu_header {
            COLOR_SIDEBAR_ROW_HOVER
        } else {
            COLOR_BACKGROUND
        };
        draw_solid_rect_i32(
            &mut frame,
            screen_w,
            screen_h,
            0,
            menu_screen_y,
            sidebar.width - 1,
            TAB_BAR_HEIGHT,
            menu_header_bg,
        );

        let menu_label = if sidebar.menu_expanded {
            "[-] Menu"
        } else {
            "[+] Menu"
        };
        let header_offset_y = (TAB_BAR_HEIGHT.saturating_sub(lh)) / 2;
        draw_string(
            &mut self.font_manager,
            &mut frame,
            menu_label,
            12,
            menu_screen_y + header_offset_y as i32,
            screen_w,
            screen_h,
            COLOR_LINE_NUMBER_ACTIVE,
        );

        if sidebar.menu_expanded {
            for (idx, (item, label)) in MENU_ITEMS.iter().enumerate() {
                let item_screen_y =
                    menu_screen_y + (TAB_BAR_HEIGHT + idx * SIDEBAR_ROW_HEIGHT) as i32;
                let is_hovered = sidebar.hovered_menu_item == Some(*item);
                let bg = if is_hovered {
                    COLOR_SIDEBAR_ROW_HOVER
                } else {
                    COLOR_BACKGROUND
                };
                draw_solid_rect_i32(
                    &mut frame,
                    screen_w,
                    screen_h,
                    0,
                    item_screen_y,
                    sidebar.width - 1,
                    SIDEBAR_ROW_HEIGHT,
                    bg,
                );
                draw_string(
                    &mut self.font_manager,
                    &mut frame,
                    label,
                    14,
                    item_screen_y + row_offset_y as i32,
                    screen_w,
                    screen_h,
                    COLOR_SIDEBAR_TEXT,
                );
            }
        }

        draw_solid_rect_i32(
            &mut frame,
            screen_w,
            screen_h,
            0,
            menu_screen_y + total_menu_h as i32 - 1,
            sidebar.width,
            1,
            COLOR_SIDEBAR_BORDER,
        );

        if sidebar.root_folder.is_some() {
            let tree_start_abs = total_menu_h;
            let active_path = tabs.active_tab().and_then(|t| t.buffer.file_path.as_ref());

            for (idx, node) in sidebar.nodes.iter().enumerate() {
                let node_screen_y =
                    (tree_start_abs + idx * SIDEBAR_ROW_HEIGHT) as i32 - sidebar.scroll_y as i32;
                if node_screen_y + (SIDEBAR_ROW_HEIGHT as i32) <= 0 {
                    continue;
                }
                if node_screen_y >= screen_h as i32 {
                    break;
                }

                let is_active = active_path == Some(&node.path);
                let is_hovered = sidebar.hovered_tree_row == Some(idx);
                let bg = if is_active {
                    COLOR_SIDEBAR_ROW_ACTIVE
                } else if is_hovered {
                    COLOR_SIDEBAR_ROW_HOVER
                } else {
                    COLOR_SIDEBAR_BG
                };

                draw_solid_rect_i32(
                    &mut frame,
                    screen_w,
                    screen_h,
                    0,
                    node_screen_y,
                    sidebar.width - 1,
                    SIDEBAR_ROW_HEIGHT,
                    bg,
                );

                let indent = 12 + (node.depth * 14);
                let prefix = if node.is_dir {
                    if node.is_expanded {
                        "[-] "
                    } else {
                        "[+] "
                    }
                } else {
                    "    "
                };
                let display_str = format!("{prefix}{}", node.name);
                let text_color = if is_active {
                    COLOR_LINE_NUMBER_ACTIVE
                } else {
                    COLOR_SIDEBAR_TEXT
                };

                draw_string_ellipsis(
                    &mut self.font_manager,
                    &mut frame,
                    &display_str,
                    indent as i32,
                    node_screen_y + row_offset_y as i32,
                    max_text_x,
                    screen_w,
                    screen_h,
                    text_color,
                );
            }
        }

        if has_sidebar_scroll {
            if let Some((thumb_y, thumb_h)) =
                calc_thumb(total_sidebar_h, screen_h, sidebar.scroll_y, screen_h)
            {
                let bar_x = sidebar.width.saturating_sub(SCROLLBAR_THICKNESS);
                draw_solid_rect(
                    &mut frame,
                    screen_w,
                    screen_h,
                    bar_x,
                    0,
                    SCROLLBAR_THICKNESS,
                    screen_h,
                    COLOR_SCROLLBAR_TRACK,
                );
                draw_solid_rect(
                    &mut frame,
                    screen_w,
                    screen_h,
                    bar_x,
                    thumb_y,
                    SCROLLBAR_THICKNESS,
                    thumb_h,
                    COLOR_SCROLLBAR_THUMB,
                );
            }
        }

        if !tabs.tabs.is_empty() {
            let tabbar_x = layout.content_left;
            let tabbar_w = screen_w.saturating_sub(tabbar_x);
            draw_solid_rect(
                &mut frame,
                screen_w,
                screen_h,
                tabbar_x,
                0,
                tabbar_w,
                TAB_BAR_HEIGHT,
                COLOR_TABBAR_BG,
            );
            draw_solid_rect(
                &mut frame,
                screen_w,
                screen_h,
                tabbar_x,
                TAB_BAR_HEIGHT - 1,
                tabbar_w,
                1,
                COLOR_TAB_BORDER,
            );

            let mut tx = tabbar_x;
            let tab_text_offset_y = (TAB_BAR_HEIGHT.saturating_sub(lh)) / 2;

            for (idx, tab) in tabs.tabs.iter().enumerate() {
                let is_active = Some(idx) == tabs.active_idx;
                let is_tab_hovered = tabs.hovered_tab == Some(idx);
                let is_close_hovered = tabs.hovered_close == Some(idx);

                let dirty = if tab.buffer.is_modified { "* " } else { "" };
                let title_text = format!("{dirty}{}", tab.title);
                let tw = (title_text.len() * cw) + 38;

                if tx + tw > screen_w {
                    break;
                }

                let bg = if is_active {
                    COLOR_TAB_ACTIVE_BG
                } else if is_tab_hovered {
                    COLOR_TAB_INACTIVE_BG
                } else {
                    COLOR_TABBAR_BG
                };

                draw_solid_rect(
                    &mut frame,
                    screen_w,
                    screen_h,
                    tx,
                    0,
                    tw,
                    TAB_BAR_HEIGHT - 1,
                    bg,
                );
                draw_solid_rect(
                    &mut frame,
                    screen_w,
                    screen_h,
                    tx + tw - 1,
                    0,
                    1,
                    TAB_BAR_HEIGHT - 1,
                    COLOR_TAB_BORDER,
                );

                let text_color = if is_active {
                    COLOR_TAB_TEXT_ACTIVE
                } else {
                    COLOR_TAB_TEXT_INACTIVE
                };
                draw_string(
                    &mut self.font_manager,
                    &mut frame,
                    &title_text,
                    tx as i32 + 10,
                    tab_text_offset_y as i32,
                    screen_w,
                    screen_h,
                    text_color,
                );

                let close_x = tx + tw - 18;
                let close_color = if is_close_hovered {
                    COLOR_TAB_CLOSE_HOVER
                } else {
                    text_color
                };
                draw_string(
                    &mut self.font_manager,
                    &mut frame,
                    "x",
                    close_x as i32,
                    tab_text_offset_y as i32,
                    screen_w,
                    screen_h,
                    close_color,
                );

                tx += tw;
            }
        }

        if let Some(tab) = tabs.active_tab() {
            let buffer = &tab.buffer;
            let (cur_line, cur_col) = buffer.cursor_pos();
            let sel_range = buffer.selection_range();

            let gutter_x = layout.content_left;
            let gutter_h = screen_h.saturating_sub(TAB_BAR_HEIGHT);
            draw_solid_rect(
                &mut frame,
                screen_w,
                screen_h,
                gutter_x,
                TAB_BAR_HEIGHT,
                layout.gutter_width,
                gutter_h,
                COLOR_GUTTER_BACKGROUND,
            );
            draw_solid_rect(
                &mut frame,
                screen_w,
                screen_h,
                gutter_x + layout.gutter_width,
                TAB_BAR_HEIGHT,
                1,
                gutter_h,
                COLOR_GUTTER_SEPARATOR,
            );

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
                    self.font_manager.draw_char(
                        &mut frame, ch, nx as i32, y as i32, screen_w, screen_h, num_color,
                    );
                    nx += cw;
                }

                let line = buffer.text().line(line_idx);
                let line_start_char = buffer.text().line_to_char(line_idx);

                for (col_idx, ch) in line.chars().enumerate() {
                    if ch == '\n' || ch == '\r' {
                        break;
                    }
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
                                &mut frame,
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

                    self.font_manager.draw_char(
                        &mut frame,
                        ch,
                        text_x as i32,
                        y as i32,
                        screen_w,
                        screen_h,
                        COLOR_TEXT_DEFAULT,
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
                    &mut frame,
                    screen_w,
                    screen_h,
                    layout.content_right,
                    TAB_BAR_HEIGHT,
                    SCROLLBAR_THICKNESS,
                    usable_track_h,
                    COLOR_SCROLLBAR_TRACK,
                );
                draw_solid_rect(
                    &mut frame,
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
                    &mut frame,
                    screen_w,
                    screen_h,
                    layout.bar_start_x,
                    layout.content_bottom,
                    horiz_track_w,
                    SCROLLBAR_THICKNESS,
                    COLOR_SCROLLBAR_TRACK,
                );
                draw_solid_rect(
                    &mut frame,
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
                &mut frame,
                screen_w,
                screen_h,
                layout.content_right,
                layout.content_bottom,
                SCROLLBAR_THICKNESS,
                SCROLLBAR_THICKNESS,
                COLOR_SCROLLBAR_TRACK,
            );
        }

        if let Some(close_idx) = tabs.pending_close {
            if let Some(tab) = tabs.tabs.get(close_idx) {
                let modal_w = 400;
                let modal_h = 130;
                let modal_x = (screen_w.saturating_sub(modal_w)) / 2;
                let modal_y = (screen_h.saturating_sub(modal_h)) / 2;

                draw_solid_rect(
                    &mut frame,
                    screen_w,
                    screen_h,
                    modal_x,
                    modal_y,
                    modal_w,
                    modal_h,
                    COLOR_MODAL_BG,
                );
                draw_solid_rect(
                    &mut frame,
                    screen_w,
                    screen_h,
                    modal_x,
                    modal_y,
                    modal_w,
                    1,
                    COLOR_MODAL_BORDER,
                );
                draw_solid_rect(
                    &mut frame,
                    screen_w,
                    screen_h,
                    modal_x,
                    modal_y + modal_h - 1,
                    modal_w,
                    1,
                    COLOR_MODAL_BORDER,
                );
                draw_solid_rect(
                    &mut frame,
                    screen_w,
                    screen_h,
                    modal_x,
                    modal_y,
                    1,
                    modal_h,
                    COLOR_MODAL_BORDER,
                );
                draw_solid_rect(
                    &mut frame,
                    screen_w,
                    screen_h,
                    modal_x + modal_w - 1,
                    modal_y,
                    1,
                    modal_h,
                    COLOR_MODAL_BORDER,
                );

                let msg = format!("Save changes to \"{}\"?", tab.title);
                draw_string(
                    &mut self.font_manager,
                    &mut frame,
                    &msg,
                    modal_x as i32 + 20,
                    modal_y as i32 + 24,
                    screen_w,
                    screen_h,
                    COLOR_LINE_NUMBER_ACTIVE,
                );

                let btn_y = modal_y + 76;
                let btns = [
                    (0, "Save", modal_x + 130, 75, COLOR_BTN_BG),
                    (1, "Discard", modal_x + 215, 85, COLOR_BTN_DANGER),
                    (2, "Cancel", modal_x + 310, 75, COLOR_BTN_BG),
                ];

                for (id, label, bx, bw, normal_bg) in btns {
                    let is_hovered = tabs.hovered_modal_btn == Some(id);
                    let bg = if is_hovered {
                        COLOR_BTN_HOVER
                    } else {
                        normal_bg
                    };
                    draw_solid_rect(&mut frame, screen_w, screen_h, bx, btn_y, bw, 28, bg);
                    let tx = bx + (bw.saturating_sub(label.len() * cw)) / 2;
                    draw_string(
                        &mut self.font_manager,
                        &mut frame,
                        label,
                        tx as i32,
                        btn_y as i32 + 6,
                        screen_w,
                        screen_h,
                        COLOR_TAB_TEXT_ACTIVE,
                    );
                }
            }
        }

        frame.present().expect("Failed to present frame");
    }
}

#[inline(always)]
fn draw_string_ellipsis(
    fonts: &mut FontManager,
    frame: &mut [u32],
    text: &str,
    start_x: i32,
    start_y: i32,
    max_x: usize,
    screen_w: usize,
    screen_h: usize,
    color: u32,
) {
    if start_x >= max_x as i32 {
        return;
    }
    let cw = fonts.char_width;
    if cw == 0 {
        return;
    }
    let avail_w = (max_x as i32 - start_x).max(0) as usize;
    let max_chars = avail_w / cw;
    let char_count = text.chars().count();

    if char_count <= max_chars {
        draw_string(
            fonts, frame, text, start_x, start_y, screen_w, screen_h, color,
        );
    } else if max_chars > 3 {
        let mut s: String = text.chars().take(max_chars - 3).collect();
        s.push_str("...");
        draw_string(
            fonts, frame, &s, start_x, start_y, screen_w, screen_h, color,
        );
    } else if max_chars > 0 {
        let s: String = text.chars().take(max_chars).collect();
        draw_string(
            fonts, frame, &s, start_x, start_y, screen_w, screen_h, color,
        );
    }
}

#[inline(always)]
fn draw_string(
    fonts: &mut FontManager,
    frame: &mut [u32],
    text: &str,
    start_x: i32,
    start_y: i32,
    screen_w: usize,
    screen_h: usize,
    color: u32,
) {
    let mut x = start_x;
    let cw = fonts.char_width as i32;
    for ch in text.chars() {
        if x >= screen_w as i32 {
            break;
        }
        fonts.draw_char(frame, ch, x, start_y, screen_w, screen_h, color);
        x += cw;
    }
}

#[inline(always)]
fn draw_solid_rect_i32(
    buf: &mut [u32],
    screen_w: usize,
    screen_h: usize,
    x: i32,
    y: i32,
    w: usize,
    h: usize,
    color: u32,
) {
    if x >= screen_w as i32 || y >= screen_h as i32 {
        return;
    }
    let x0 = x.max(0) as usize;
    let y0 = y.max(0) as usize;
    let x1 = ((x + w as i32).max(0) as usize).min(screen_w);
    let y1 = ((y + h as i32).max(0) as usize).min(screen_h);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for row in y0..y1 {
        let offset = row * screen_w;
        buf[offset + x0..offset + x1].fill(color);
    }
}

#[inline(always)]
fn draw_solid_rect(
    buf: &mut [u32],
    screen_w: usize,
    screen_h: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    let start_y = y.min(screen_h);
    let end_y = (y + h).min(screen_h);
    let start_x = x.min(screen_w);
    let end_x = (x + w).min(screen_w);
    if start_x >= end_x {
        return;
    }
    for row in start_y..end_y {
        let offset = row * screen_w;
        buf[offset + start_x..offset + end_x].fill(color);
    }
}
