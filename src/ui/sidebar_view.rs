use super::canvas::{draw_solid_rect, draw_solid_rect_i32, draw_string, draw_string_ellipsis};
use super::font::FontManager;
use crate::config::*;
use crate::editor::TabManager;
use crate::sidebar::{MenuItem, Sidebar};
use crate::terminal::Terminal;
use crate::ui::layout::calc_thumb;

pub fn render_sidebar(
    frame: &mut [u32],
    fonts: &mut FontManager,
    sidebar: &Sidebar,
    tabs: &TabManager,
    terminal: &Terminal,
    screen_w: usize,
    screen_h: usize,
) {
    if !sidebar.visible {
        return;
    }

    let lh = fonts.line_height;
    let row_offset_y = (SIDEBAR_ROW_HEIGHT.saturating_sub(lh)) / 2;

    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        0,
        0,
        sidebar.width,
        screen_h,
        COLOR_SIDEBAR_BG,
    );
    draw_solid_rect(
        frame,
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
        frame,
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
        frame,
        screen_w,
        screen_h,
        0,
        menu_screen_y,
        sidebar.width - 1,
        TAB_BAR_HEIGHT,
        menu_header_bg,
    );

    let menu_label = if sidebar.menu_expanded {
        "[-] MENU"
    } else {
        "[+] MENU"
    };
    let header_offset_y = (TAB_BAR_HEIGHT.saturating_sub(lh)) / 2;
    draw_string(
        fonts,
        frame,
        menu_label,
        12,
        menu_screen_y + header_offset_y as i32,
        screen_w,
        screen_h,
        COLOR_LINE_NUMBER_ACTIVE,
    );

    let can_save = tabs
        .active_tab()
        .map(|t| t.buffer.is_modified)
        .unwrap_or(false);
    let has_folder = sidebar.root_folder.is_some();

    if sidebar.menu_expanded {
        for (idx, (item, label)) in sidebar.menu_items().iter().enumerate() {
            let item_screen_y = menu_screen_y + (TAB_BAR_HEIGHT + idx * SIDEBAR_ROW_HEIGHT) as i32;
            let is_disabled = (*item == MenuItem::Save && !can_save)
                || (*item == MenuItem::CloseFolder && !has_folder);
            let is_hovered = sidebar.hovered_menu_item == Some(*item);
            let bg = if is_hovered && !is_disabled {
                COLOR_SIDEBAR_ROW_HOVER
            } else {
                COLOR_BACKGROUND
            };
            draw_solid_rect_i32(
                frame,
                screen_w,
                screen_h,
                0,
                item_screen_y,
                sidebar.width - 1,
                SIDEBAR_ROW_HEIGHT,
                bg,
            );
            let text_color = if is_disabled {
                COLOR_LINE_NUMBER_MUTED
            } else {
                COLOR_SIDEBAR_TEXT
            };
            draw_string(
                fonts,
                frame,
                label,
                14,
                item_screen_y + row_offset_y as i32,
                screen_w,
                screen_h,
                text_color,
            );
        }
    }

    draw_solid_rect_i32(
        frame,
        screen_w,
        screen_h,
        0,
        menu_screen_y + total_menu_h as i32 - 1,
        sidebar.width,
        1,
        COLOR_SIDEBAR_BORDER,
    );

    let term_screen_y = menu_screen_y + total_menu_h as i32;
    let term_header_bg = if sidebar.hovered_terminal_header {
        COLOR_SIDEBAR_ROW_HOVER
    } else {
        COLOR_BACKGROUND
    };
    draw_solid_rect_i32(
        frame,
        screen_w,
        screen_h,
        0,
        term_screen_y,
        sidebar.width - 1,
        TAB_BAR_HEIGHT,
        term_header_bg,
    );
    let term_label = if terminal.is_open {
        "[-] TERMINAL"
    } else {
        "[+] TERMINAL"
    };
    draw_string(
        fonts,
        frame,
        term_label,
        12,
        term_screen_y + header_offset_y as i32,
        screen_w,
        screen_h,
        COLOR_LINE_NUMBER_ACTIVE,
    );
    draw_solid_rect_i32(
        frame,
        screen_w,
        screen_h,
        0,
        term_screen_y + TAB_BAR_HEIGHT as i32 - 1,
        sidebar.width,
        1,
        COLOR_SIDEBAR_BORDER,
    );

    if sidebar.root_folder.is_some() {
        let root_screen_y = term_screen_y + TAB_BAR_HEIGHT as i32;
        let root_name = sidebar.root_name().unwrap_or_else(|| "FOLDER".to_string());
        let root_prefix = if sidebar.root_expanded {
            "[-] "
        } else {
            "[+] "
        };
        let root_label = format!("{root_prefix}{root_name}");

        if root_screen_y + (TAB_BAR_HEIGHT as i32) > 0 && root_screen_y < screen_h as i32 {
            let is_root_hovered = sidebar.hovered_root_header;
            let root_bg = if is_root_hovered {
                COLOR_SIDEBAR_ROW_HOVER
            } else {
                COLOR_SIDEBAR_BG
            };

            draw_solid_rect_i32(
                frame,
                screen_w,
                screen_h,
                0,
                root_screen_y,
                sidebar.width - 1,
                TAB_BAR_HEIGHT,
                root_bg,
            );

            draw_string_ellipsis(
                fonts,
                frame,
                &root_label,
                12,
                root_screen_y + header_offset_y as i32,
                max_text_x,
                screen_w,
                screen_h,
                COLOR_LINE_NUMBER_ACTIVE,
            );
        }

        if sidebar.root_expanded {
            let tree_start_abs = total_menu_h + TAB_BAR_HEIGHT + TAB_BAR_HEIGHT;
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
                    frame,
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
                    ""
                };
                let display_str = format!("{prefix}{}", node.name);
                let text_color = if is_active {
                    COLOR_LINE_NUMBER_ACTIVE
                } else {
                    COLOR_SIDEBAR_TEXT
                };

                draw_string_ellipsis(
                    fonts,
                    frame,
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
    }

    if has_sidebar_scroll {
        if let Some((thumb_y, thumb_h)) =
            calc_thumb(total_sidebar_h, screen_h, sidebar.scroll_y, screen_h)
        {
            let bar_x = sidebar.width.saturating_sub(SCROLLBAR_THICKNESS);
            draw_solid_rect(
                frame,
                screen_w,
                screen_h,
                bar_x,
                0,
                SCROLLBAR_THICKNESS,
                screen_h,
                COLOR_SCROLLBAR_TRACK,
            );
            draw_solid_rect(
                frame,
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
}
