use super::canvas::{draw_solid_rect, draw_string};
use super::font::FontManager;
use crate::config::*;
use crate::editor::TabManager;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ContextMenu {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub hovered_idx: Option<usize>,
}

pub fn render_context_menu(
    frame: &mut [u32],
    fonts: &mut FontManager,
    menu: &ContextMenu,
    sidebar_visible: bool,
    tab_count: usize,
    screen_w: usize,
    screen_h: usize,
) {
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        menu.x,
        menu.y,
        menu.width,
        menu.height,
        COLOR_MODAL_BG,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        menu.x,
        menu.y,
        menu.width,
        1,
        COLOR_MODAL_BORDER,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        menu.x,
        menu.y + menu.height - 1,
        menu.width,
        1,
        COLOR_MODAL_BORDER,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        menu.x,
        menu.y,
        1,
        menu.height,
        COLOR_MODAL_BORDER,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        menu.x + menu.width - 1,
        menu.y,
        1,
        menu.height,
        COLOR_MODAL_BORDER,
    );
    let row_h = menu.height / 2;
    let sidebar_label = if sidebar_visible {
        "Close Sidebar"
    } else {
        "Open Sidebar"
    };
    let close_files_label = if tab_count <= 1 {
        "Close File"
    } else {
        "Close Files"
    };
    let has_files = tab_count > 0;
    let items = [(sidebar_label, true), (close_files_label, has_files)];
    let cap_h = (fonts.baseline_offset * 73) / 100;
    for (i, (label, enabled)) in items.iter().enumerate() {
        let item_y = menu.y + i * row_h;
        if *enabled && menu.hovered_idx == Some(i) {
            draw_solid_rect(
                frame,
                screen_w,
                screen_h,
                menu.x + 1,
                item_y + 1,
                menu.width.saturating_sub(2),
                row_h.saturating_sub(1),
                COLOR_SIDEBAR_ROW_HOVER,
            );
        }
        let ty = item_y as i32 + (row_h as i32 + cap_h as i32) / 2 - fonts.baseline_offset as i32;
        let text_color = if *enabled {
            COLOR_TAB_TEXT_ACTIVE
        } else {
            COLOR_LINE_NUMBER_MUTED
        };
        draw_string(
            fonts,
            frame,
            label,
            menu.x as i32 + 12,
            ty,
            screen_w,
            screen_h,
            text_color,
        );
    }
}

pub struct ModalButton {
    pub id: usize,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub label: &'static str,
    pub is_danger: bool,
}

pub struct ModalLayout {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub text_lines: Vec<(String, usize, usize, u32)>,
    pub buttons: Vec<ModalButton>,
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut lines = Vec::new();
    let mut current_line = String::new();
    for word in text.split_whitespace() {
        if current_line.is_empty() {
            if word.chars().count() <= max_chars {
                current_line.push_str(word);
            } else {
                let mut chunk = String::new();
                for ch in word.chars() {
                    chunk.push(ch);
                    if chunk.chars().count() == max_chars {
                        lines.push(std::mem::take(&mut chunk));
                    }
                }
                if !chunk.is_empty() {
                    current_line = chunk;
                }
            }
        } else if current_line.chars().count() + 1 + word.chars().count() <= max_chars {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current_line));
            if word.chars().count() <= max_chars {
                current_line.push_str(word);
            } else {
                let mut chunk = String::new();
                for ch in word.chars() {
                    chunk.push(ch);
                    if chunk.chars().count() == max_chars {
                        lines.push(std::mem::take(&mut chunk));
                    }
                }
                if !chunk.is_empty() {
                    current_line = chunk;
                }
            }
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub fn compute_modal_layout(
    tabs: &TabManager,
    screen_w: usize,
    screen_h: usize,
    char_w: usize,
    line_h: usize,
) -> Option<ModalLayout> {
    if !tabs.closing_app && !tabs.closing_files && tabs.pending_close.is_none() {
        return None;
    }
    let is_multi = if tabs.closing_app || tabs.closing_files {
        tabs.tabs.iter().filter(|t| t.buffer.is_modified).count() > 1
    } else {
        false
    };
    let btn_save_label = if is_multi { "Save All" } else { "Save" };
    let btn_discard_label = if is_multi { "Discard All" } else { "Discard" };
    let btn_cancel_label = "Cancel";
    let title_line = "Save changes before closing?".to_string();
    let pad_x = 20;
    let pad_y = 16;
    let modal_w = 440.min(screen_w.saturating_sub(16)).max(120);
    let modal_x = (screen_w.saturating_sub(modal_w)) / 2;
    let content_w = modal_w.saturating_sub(pad_x * 2).max(char_w);
    let max_chars = if char_w > 0 { content_w / char_w } else { 20 };
    let title_lines = wrap_text(&title_line, max_chars);
    let btn_h = 28;
    let spacing = 8;
    let w_save = (btn_save_label.chars().count() * char_w + 16).max(60);
    let w_discard = (btn_discard_label.chars().count() * char_w + 16).max(60);
    let w_cancel = (btn_cancel_label.chars().count() * char_w + 16).max(60);
    let total_horiz_btn_w = w_save + w_discard + w_cancel + spacing * 2;
    let can_fit_single_row = total_horiz_btn_w <= content_w;
    let btn_area_h = if can_fit_single_row {
        btn_h
    } else {
        btn_h * 3 + spacing * 2
    };
    let text_lines_h = title_lines.len() * (line_h + 2);
    let raw_modal_h = pad_y + text_lines_h + 16 + btn_area_h + pad_y;
    let modal_h = raw_modal_h.min(screen_h.saturating_sub(8)).max(60);
    let modal_y = (screen_h.saturating_sub(modal_h)) / 2;
    let mut text_lines = Vec::new();
    let mut cur_y = modal_y + pad_y;
    for line in title_lines {
        if cur_y + line_h <= modal_y + modal_h.saturating_sub(btn_area_h + pad_y) {
            text_lines.push((line, modal_x + pad_x, cur_y, COLOR_TAB_TEXT_ACTIVE));
        }
        cur_y += line_h + 2;
    }
    let mut buttons = Vec::new();
    if can_fit_single_row {
        let btns_y = modal_y + modal_h.saturating_sub(btn_h + pad_y);
        let start_btn_x = modal_x + pad_x + content_w.saturating_sub(total_horiz_btn_w);
        buttons.push(ModalButton {
            id: 0,
            x: start_btn_x,
            y: btns_y,
            w: w_save,
            h: btn_h,
            label: btn_save_label,
            is_danger: false,
        });
        buttons.push(ModalButton {
            id: 1,
            x: start_btn_x + w_save + spacing,
            y: btns_y,
            w: w_discard,
            h: btn_h,
            label: btn_discard_label,
            is_danger: true,
        });
        buttons.push(ModalButton {
            id: 2,
            x: start_btn_x + w_save + spacing + w_discard + spacing,
            y: btns_y,
            w: w_cancel,
            h: btn_h,
            label: btn_cancel_label,
            is_danger: false,
        });
    } else {
        let base_y = modal_y + modal_h.saturating_sub(btn_area_h + pad_y);
        let b_w = content_w;
        buttons.push(ModalButton {
            id: 0,
            x: modal_x + pad_x,
            y: base_y,
            w: b_w,
            h: btn_h,
            label: btn_save_label,
            is_danger: false,
        });
        buttons.push(ModalButton {
            id: 1,
            x: modal_x + pad_x,
            y: base_y + btn_h + spacing,
            w: b_w,
            h: btn_h,
            label: btn_discard_label,
            is_danger: true,
        });
        buttons.push(ModalButton {
            id: 2,
            x: modal_x + pad_x,
            y: base_y + (btn_h + spacing) * 2,
            w: b_w,
            h: btn_h,
            label: btn_cancel_label,
            is_danger: false,
        });
    }
    Some(ModalLayout {
        x: modal_x,
        y: modal_y,
        w: modal_w,
        h: modal_h,
        text_lines,
        buttons,
    })
}

pub fn render_modal(
    frame: &mut [u32],
    fonts: &mut FontManager,
    modal: &ModalLayout,
    hovered_modal_btn: Option<usize>,
    screen_w: usize,
    screen_h: usize,
) {
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        modal.x,
        modal.y,
        modal.w,
        modal.h,
        COLOR_MODAL_BG,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        modal.x,
        modal.y,
        modal.w,
        1,
        COLOR_MODAL_BORDER,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        modal.x,
        modal.y + modal.h - 1,
        modal.w,
        1,
        COLOR_MODAL_BORDER,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        modal.x,
        modal.y,
        1,
        modal.h,
        COLOR_MODAL_BORDER,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        modal.x + modal.w - 1,
        modal.y,
        1,
        modal.h,
        COLOR_MODAL_BORDER,
    );
    for (text, tx, ty, color) in &modal.text_lines {
        draw_string(
            fonts, frame, text, *tx as i32, *ty as i32, screen_w, screen_h, *color,
        );
    }
    let cap_h = (fonts.baseline_offset * 73) / 100;
    for btn in &modal.buttons {
        let is_hovered = hovered_modal_btn == Some(btn.id);
        let bg = if is_hovered {
            COLOR_BTN_HOVER
        } else if btn.is_danger {
            COLOR_BTN_DANGER
        } else {
            COLOR_BTN_BG
        };
        draw_solid_rect(frame, screen_w, screen_h, btn.x, btn.y, btn.w, btn.h, bg);
        let text_w = btn.label.chars().count() * fonts.char_width;
        let tx = btn.x + (btn.w.saturating_sub(text_w)) / 2;
        let ty = btn.y as i32 + (btn.h as i32 + cap_h as i32) / 2 - fonts.baseline_offset as i32;
        draw_string(
            fonts,
            frame,
            btn.label,
            tx as i32,
            ty,
            screen_w,
            screen_h,
            COLOR_TAB_TEXT_ACTIVE,
        );
    }
}
