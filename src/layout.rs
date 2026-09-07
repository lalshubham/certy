use crate::config::*;
use crate::tabs::TabManager;

pub struct ViewportLayout {
    pub content_left: usize,
    pub content_right: usize,
    pub content_bottom: usize,
    pub gutter_width: usize,
    pub code_x: usize,
    pub bar_start_x: usize,
    pub visible_lines: usize,
    pub visible_cols: usize,
}

pub fn calc_thumb(
    total: usize,
    visible: usize,
    offset: usize,
    track_len: usize,
) -> Option<(usize, usize)> {
    if total <= visible || track_len == 0 {
        return None;
    }
    let ratio = visible as f64 / total as f64;
    let thumb_h = ((track_len as f64 * ratio) as usize).clamp(MIN_THUMB_SIZE, track_len);
    let max_offset = total - visible;
    let travel = track_len.saturating_sub(thumb_h);
    let pos = ((offset as f64 / max_offset as f64) * travel as f64) as usize;
    Some((pos, thumb_h))
}

pub fn compute_layout(
    screen_w: usize,
    effective_h: usize,
    char_w: usize,
    line_h: usize,
    total_lines: usize,
    sidebar_w: usize,
) -> ViewportLayout {
    let content_left = sidebar_w;
    let content_right = screen_w.saturating_sub(SCROLLBAR_THICKNESS);
    let content_bottom = effective_h.saturating_sub(SCROLLBAR_THICKNESS);

    let digits = total_lines.to_string().len().max(3);
    let gutter_width = GUTTER_PADDING * 2 + digits * char_w;
    let code_x = content_left + gutter_width + CODE_LEFT_MARGIN;
    let bar_start_x = content_left + gutter_width + 1;

    let code_w = content_right.saturating_sub(code_x);
    let code_h = content_bottom.saturating_sub(TAB_BAR_HEIGHT + TOP_PADDING);

    let visible_cols = if char_w > 0 { code_w / char_w } else { 0 };
    let visible_lines = if line_h > 0 { code_h / line_h } else { 0 };

    ViewportLayout {
        content_left,
        content_right,
        content_bottom,
        gutter_width,
        code_x,
        bar_start_x,
        visible_lines,
        visible_cols,
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

    let title_line = if is_multi {
        "Save changes to modified files before closing?".to_string()
    } else if let Some(idx) = tabs.pending_close {
        if let Some(tab) = tabs.tabs.get(idx) {
            format!("Save changes to \"{}\" before closing?", tab.title)
        } else {
            "Save changes before closing?".to_string()
        }
    } else if let Some(tab) = tabs.tabs.iter().find(|t| t.buffer.is_modified) {
        format!("Save changes to \"{}\" before closing?", tab.title)
    } else {
        "Save changes before closing?".to_string()
    };

    let sub_line = "Your changes will be lost if you don't save them.".to_string();

    let pad_x = 20;
    let pad_y = 16;
    let modal_w = 440.min(screen_w.saturating_sub(16)).max(120);
    let modal_x = (screen_w.saturating_sub(modal_w)) / 2;

    let content_w = modal_w.saturating_sub(pad_x * 2).max(char_w);
    let max_chars = if char_w > 0 { content_w / char_w } else { 20 };

    let title_lines = wrap_text(&title_line, max_chars);
    let sub_lines = wrap_text(&sub_line, max_chars);

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

    let text_lines_h = title_lines.len() * (line_h + 2) + 6 + sub_lines.len() * (line_h + 2);
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

    cur_y += 6;
    for line in sub_lines {
        if cur_y + line_h <= modal_y + modal_h.saturating_sub(btn_area_h + pad_y) {
            text_lines.push((line, modal_x + pad_x, cur_y, COLOR_LINE_NUMBER_ACTIVE));
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
