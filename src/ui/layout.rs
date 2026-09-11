use crate::config::*;

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
    let digits = if total_lines == 0 {
        1
    } else {
        (total_lines.ilog10() + 1) as usize
    }
    .max(3);
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
