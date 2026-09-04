use crate::config::*;

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
    let available = track_len.saturating_sub(thumb_size);
    let offset = (ratio * available as f64) as usize;
    Some((offset, thumb_size))
}
