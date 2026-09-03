use crate::config::*;

pub struct ViewportLayout {
    pub gutter_width: usize,
    pub code_x: usize,
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

    let visible_lines = screen_h.saturating_sub(TOP_PADDING + SCROLLBAR_THICKNESS) / line_h;
    let visible_cols = content_right.saturating_sub(code_x) / char_w;

    ViewportLayout {
        gutter_width,
        code_x,
        content_right,
        content_bottom,
        visible_lines,
        visible_cols,
    }
}

pub fn calc_vert_thumb(
    total_lines: usize,
    vis_lines: usize,
    scroll_line: usize,
    track_height: usize,
) -> Option<(usize, usize)> {
    if total_lines <= vis_lines || vis_lines == 0 || track_height == 0 {
        return None;
    }
    let thumb_h = ((vis_lines as f64 / total_lines as f64) * track_height as f64)
        .clamp(MIN_THUMB_SIZE as f64, track_height as f64) as usize;
    let max_scroll = total_lines.saturating_sub(vis_lines);
    if max_scroll == 0 {
        return None;
    }
    let ratio = (scroll_line as f64 / max_scroll as f64).clamp(0.0, 1.0);
    let available_track = track_height.saturating_sub(thumb_h);
    let thumb_y = (ratio * available_track as f64) as usize;
    Some((thumb_y, thumb_h))
}

pub fn calc_horiz_thumb(
    max_cols: usize,
    vis_cols: usize,
    scroll_col: usize,
    bar_start_x: usize,
    content_right: usize,
) -> Option<(usize, usize)> {
    if max_cols <= vis_cols || vis_cols == 0 || content_right <= bar_start_x {
        return None;
    }
    let track_width = content_right - bar_start_x;
    let thumb_w = ((vis_cols as f64 / max_cols as f64) * track_width as f64)
        .clamp(MIN_THUMB_SIZE as f64, track_width as f64) as usize;
    let max_scroll = max_cols.saturating_sub(vis_cols);
    if max_scroll == 0 {
        return None;
    }
    let ratio = (scroll_col as f64 / max_scroll as f64).clamp(0.0, 1.0);
    let available_track = track_width.saturating_sub(thumb_w);
    let thumb_x = bar_start_x + (ratio * available_track as f64) as usize;
    Some((thumb_x, thumb_w))
}
