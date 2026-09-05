use crate::config::*;
use crate::tabs::TabManager;

pub struct ViewportLayout {
    pub content_left: usize,
    pub gutter_width: usize,
    pub code_x: usize,
    pub bar_start_x: usize,
    pub content_right: usize,
    pub content_bottom: usize,
    pub visible_lines: usize,
    pub visible_cols: usize,
}

pub fn compute_layout(
    screen_w: usize,
    screen_h: usize,
    char_w: usize,
    line_h: usize,
    total_lines: usize,
    sidebar_w: usize,
) -> ViewportLayout {
    let content_left = sidebar_w;
    let content_bottom = screen_h.saturating_sub(SCROLLBAR_THICKNESS);
    let content_right = screen_w.saturating_sub(SCROLLBAR_THICKNESS);

    let digits = total_lines.to_string().len().max(3);
    let gutter_width = GUTTER_PADDING + (digits * char_w) + GUTTER_PADDING;
    let code_x = content_left + gutter_width + CODE_LEFT_MARGIN;
    let bar_start_x = content_left + gutter_width + 1;

    let usable_height = content_bottom.saturating_sub(TAB_BAR_HEIGHT + TOP_PADDING);
    let visible_lines = if line_h > 0 {
        usable_height / line_h
    } else {
        0
    };
    let visible_cols = if char_w > 0 {
        content_right.saturating_sub(code_x) / char_w
    } else {
        0
    };

    ViewportLayout {
        content_left,
        gutter_width,
        code_x,
        bar_start_x,
        content_right,
        content_bottom,
        visible_lines,
        visible_cols,
    }
}

pub fn calc_thumb(
    total: usize,
    vis: usize,
    scroll: usize,
    track_len: usize,
) -> Option<(usize, usize)> {
    if total <= vis || vis == 0 || track_len == 0 {
        return None;
    }
    let max_thumb = track_len as f64;
    let min_thumb = (MIN_THUMB_SIZE as f64).min(max_thumb);
    let thumb_size = ((vis as f64 / total as f64) * max_thumb).clamp(min_thumb, max_thumb) as usize;
    let max_scroll = total - vis;
    if max_scroll == 0 {
        return None;
    }
    let ratio = (scroll as f64 / max_scroll as f64).clamp(0.0, 1.0);
    let offset = (ratio * (track_len.saturating_sub(thumb_size)) as f64) as usize;
    Some((offset, thumb_size))
}

pub fn wrap_text(text: &str, max_cols: usize) -> Vec<String> {
    if max_cols == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_len = 0;

    for word in text.split(' ') {
        if word.is_empty() {
            continue;
        }
        let word_len = word.chars().count();
        if current_len == 0 {
            if word_len <= max_cols {
                current_line.push_str(word);
                current_len = word_len;
            } else {
                let chars: Vec<char> = word.chars().collect();
                let mut start = 0;
                while start < word_len {
                    let end = (start + max_cols).min(word_len);
                    let chunk: String = chars[start..end].iter().collect();
                    if end < word_len {
                        lines.push(chunk);
                    } else {
                        current_line = chunk;
                        current_len = end - start;
                    }
                    start = end;
                }
            }
        } else if current_len + 1 + word_len <= max_cols {
            current_line.push(' ');
            current_line.push_str(word);
            current_len += 1 + word_len;
        } else {
            lines.push(current_line);
            current_line = String::new();
            current_len = 0;
            if word_len <= max_cols {
                current_line.push_str(word);
                current_len = word_len;
            } else {
                let chars: Vec<char> = word.chars().collect();
                let mut start = 0;
                while start < word_len {
                    let end = (start + max_cols).min(word_len);
                    let chunk: String = chars[start..end].iter().collect();
                    if end < word_len {
                        lines.push(chunk);
                    } else {
                        current_line = chunk;
                        current_len = end - start;
                    }
                    start = end;
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

pub struct ModalButton {
    pub id: usize,
    pub label: &'static str,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub is_danger: bool,
}

pub struct ModalDialogLayout {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub text_lines: Vec<(String, usize, usize, u32)>,
    pub buttons: [ModalButton; 3],
}

pub fn compute_modal_layout(
    tabs: &TabManager,
    screen_w: usize,
    screen_h: usize,
    char_w: usize,
    line_h: usize,
) -> Option<ModalDialogLayout> {
    if !tabs.closing_app && tabs.pending_close.is_none() {
        return None;
    }

    let modal_w = 460.min(screen_w.saturating_sub(20)).max(320);
    let usable_w = modal_w.saturating_sub(40);
    let max_cols = if char_w > 0 { usable_w / char_w } else { 38 }.max(10);
    let line_pitch = line_h.max(16) + 4;

    let btn2_label = "Cancel";

    let (btn0_label, btn1_label, raw_lines) = if tabs.closing_app {
        let unsaved: Vec<&str> = tabs
            .tabs
            .iter()
            .filter(|t| t.buffer.is_modified)
            .map(|t| t.title.as_str())
            .collect();

        if unsaved.is_empty() {
            return None;
        }

        let is_multiple = unsaved.len() > 1;
        let btn0 = if is_multiple { "Save All" } else { "Save" };
        let btn1 = if is_multiple {
            "Discard All"
        } else {
            "Discard"
        };

        let header_text = if is_multiple {
            "Save changes to the following files before closing?"
        } else {
            "Save changes to the following file before closing?"
        };

        let mut lines: Vec<(String, u32)> = wrap_text(header_text, max_cols)
            .into_iter()
            .map(|l| (l, COLOR_LINE_NUMBER_ACTIVE))
            .collect();

        let file_cols = max_cols.saturating_sub(4).max(6);
        for name in unsaved.iter().take(6) {
            let file_chunks = wrap_text(name, file_cols);
            for (i, chunk) in file_chunks.into_iter().enumerate() {
                let formatted = if i == 0 {
                    format!("  * {chunk}")
                } else {
                    format!("    {chunk}")
                };
                lines.push((formatted, COLOR_TEXT_DEFAULT));
            }
        }

        if unsaved.len() > 6 {
            lines.push((
                format!("  ... and {} more", unsaved.len() - 6),
                COLOR_TAB_TEXT_INACTIVE,
            ));
        }

        (btn0, btn1, lines)
    } else {
        let close_idx = tabs.pending_close?;
        let tab = tabs.tabs.get(close_idx)?;

        let header_text = format!("Save changes to \"{}\" before closing?", tab.title);
        let lines: Vec<(String, u32)> = wrap_text(&header_text, max_cols)
            .into_iter()
            .map(|l| (l, COLOR_LINE_NUMBER_ACTIVE))
            .collect();

        ("Save", "Discard", lines)
    };

    let text_h = raw_lines.len() * line_pitch;
    let modal_h = 20 + text_h + 20 + 28 + 16;
    let modal_x = (screen_w.saturating_sub(modal_w)) / 2;
    let modal_y = (screen_h.saturating_sub(modal_h)) / 2;

    let positioned_lines = raw_lines
        .into_iter()
        .enumerate()
        .map(|(i, (text, color))| {
            let y = modal_y + 20 + i * line_pitch;
            let x = modal_x + 20;
            (text, x, y, color)
        })
        .collect();

    let btn_y = modal_y + modal_h.saturating_sub(16 + 28);
    let gap = 12;
    let right_margin = 20;

    let btn2_w = ((btn2_label.len() * char_w) + 24).max(75);
    let btn2_x = modal_x + modal_w.saturating_sub(right_margin + btn2_w);

    let btn1_w = ((btn1_label.len() * char_w) + 24).max(85);
    let btn1_x = btn2_x.saturating_sub(gap + btn1_w);

    let btn0_w = ((btn0_label.len() * char_w) + 24).max(75);
    let btn0_x = btn1_x.saturating_sub(gap + btn0_w);

    let buttons = [
        ModalButton {
            id: 0,
            label: btn0_label,
            x: btn0_x,
            y: btn_y,
            w: btn0_w,
            h: 28,
            is_danger: false,
        },
        ModalButton {
            id: 1,
            label: btn1_label,
            x: btn1_x,
            y: btn_y,
            w: btn1_w,
            h: 28,
            is_danger: true,
        },
        ModalButton {
            id: 2,
            label: btn2_label,
            x: btn2_x,
            y: btn_y,
            w: btn2_w,
            h: 28,
            is_danger: false,
        },
    ];

    Some(ModalDialogLayout {
        x: modal_x,
        y: modal_y,
        w: modal_w,
        h: modal_h,
        text_lines: positioned_lines,
        buttons,
    })
}
