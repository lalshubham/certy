use crate::config::*;

pub struct ViewportLayout {
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
) -> ViewportLayout {
    let content_bottom = screen_h.saturating_sub(SCROLLBAR_THICKNESS);
    let content_right = screen_w.saturating_sub(SCROLLBAR_THICKNESS);

    let digits = total_lines.to_string().len().max(3);
    let gutter_width = GUTTER_PADDING + (digits * char_w) + GUTTER_PADDING;
    let code_x = gutter_width + CODE_LEFT_MARGIN;
    let bar_start_x = gutter_width + 1;

    let visible_lines = screen_h.saturating_sub(TOP_PADDING + SCROLLBAR_THICKNESS) / line_h;
    let visible_cols = content_right.saturating_sub(code_x) / char_w;

    ViewportLayout {
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
    visible: usize,
    scroll: usize,
    track_len: usize,
) -> Option<(usize, usize)> {
    if total <= visible || visible == 0 || track_len == 0 {
        return None;
    }
    let thumb_size = ((visible as f64 / total as f64) * track_len as f64)
        .clamp(MIN_THUMB_SIZE as f64, track_len as f64) as usize;
    let max_scroll = total - visible;
    if max_scroll == 0 {
        return None;
    }
    let ratio = (scroll as f64 / max_scroll as f64).clamp(0.0, 1.0);
    let available = track_len.saturating_sub(thumb_size);
    let offset = (ratio * available as f64) as usize;
    Some((offset, thumb_size))
}
